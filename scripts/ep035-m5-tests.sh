#!/usr/bin/env sh
# EP-035 M5 gate: one-package deployment live-fire (LF-001) + closure
# proofs.
#
# Runs the REAL LF-001 one-package-deployment journey through the
# deployment live-fire package (tests/livefire/deployment) against REAL
# ephemeral PostgreSQL 18.4 + NATS 2.14.3 containers (digest-pinned per
# COMPONENT_REGISTRY.yaml), with vacuity guards, production-import
# guards, anti-masking sentinels, current-run evidence freshness, LF
# runner integrity, M1-M4 regressions, and orphan/resource hygiene.
#
# LF-001 proves the exact owned one-package journey (source commit ->
# one-package bundle -> clean ephemeral target -> package DDL boot ->
# real readiness -> deployment intent -> owner bootstrap -> exact-target
# readback -> verification evidence -> redacted events -> idempotent
# replay -> current-run evidence). M3/M4 are required regressions, NOT
# substitutes for LF-001.
set -eu
export CI=true

log="/tmp/ep035-m5-tests.log"
: > "$log"

fail() {
  echo "EP-035 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-035 M5 gate: $1"; }

PNPM="${PNPM_BIN:-pnpm}"
PKG="tests/livefire/deployment"

# Vacuity guard 0: the live-fire package must exist.
if [ ! -f "$PKG/package.json" ]; then
  fail "$PKG/package.json missing"
fi
ok "live-fire package present"

# Vacuity guard 0b: all owned live-fire sources must exist.
for f in \
  src/__tests__/harness.ts \
  src/__tests__/ep035_lf001_one_package.test.ts; do
  if [ ! -f "$PKG/$f" ]; then
    fail "$PKG/$f missing"
  fi
done
if [ ! -f scripts/ep035-one-package-build.sh ]; then
  fail "scripts/ep035-one-package-build.sh missing"
fi
ok "owned live-fire sources and bundle builder present"

# Vacuity guard 0c: the M5 fence must be populated (not a placeholder).
if grep -q "Populated at M5" .agent/milestone-files/EP-035-M5.txt; then
  fail "M5 fence is still a placeholder comment"
fi
ok "M5 fence populated"

# Production-import guard: the journey composes REAL production
# components and never substitutes a mock-only clone.
if ! grep -q '@nexus/onboarding' "$PKG/src/__tests__/ep035_lf001_one_package.test.ts"; then
  fail "LF-001 proof does not import production @nexus/onboarding"
fi
if ! grep -q '@nexus/setup' "$PKG/src/__tests__/ep035_lf001_one_package.test.ts"; then
  fail "LF-001 proof does not import production @nexus/setup"
fi
if ! grep -q 'OwnerBootstrapStore' "$PKG/src/__tests__/ep035_lf001_one_package.test.ts"; then
  fail "LF-001 proof does not use the production OwnerBootstrapStore"
fi
if ! grep -q 'DeploymentIntentStore' "$PKG/src/__tests__/ep035_lf001_one_package.test.ts"; then
  fail "LF-001 proof does not use the production DeploymentIntentStore"
fi
if ! grep -q 'OnboardingEventPublisher' "$PKG/src/__tests__/ep035_lf001_one_package.test.ts"; then
  fail "LF-001 proof does not use the production OnboardingEventPublisher"
fi
if grep -rq 'vi.mock\|mock(' "$PKG/src/__tests__"; then
  fail "mock-only substitute detected in live-fire suite"
fi
ok "production components imported; no mock-only substitute"

# Real typecheck.
if ! (cd "$PKG" && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Real test run: the full one-package deployment journey, file-serial
# (the suite spawns its own postgres+nats containers; parallel startup
# under load caused NATS subscriber timeouts - M3 lesson). Verbose
# reporter so every owned proof name is observable in the log.
if ! (cd "$PKG" && "$PNPM" exec vitest run src/__tests__ --no-file-parallelism --reporter=verbose >>"$log" 2>&1); then
  fail "vitest run failed" "$log"
fi
sed -i 's/\x1b\[[0-9;]*m//g' "$log"

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'Tests  +[1-9][0-9]* passed' "$log"; then
  fail "zero tests passed (vacuity)" "$log"
fi
# Vacuity guard 2: zero failures observed.
if grep -qE '[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi
if grep -qE 'Tests  +[0-9]+ skipped' "$log"; then
  fail "skipped tests present (vacuity guard)" "$log"
fi
# Vacuity guard 3: the EP-035-owned live-fire suite actually ran.
if ! grep -q 'ep035_lf001_one_package_deployment' "$log"; then
  fail "EP-035-owned live-fire suite did not run (anti-masking guard)" "$log"
fi
# Vacuity guard 4: exact owned proof names observed (anti-masking).
for sentinel in \
  "binds the one-package artifact identity to the current commit" \
  "boots the clean target and observes real runtime readiness" \
  "records deployment selection as intent (local provider profile)" \
  "bootstraps the first owner with exact-target readback" \
  "requires evidence to become VERIFIED (SELECTED != VERIFIED)" \
  "emits redacted owner and deployment events over the real bus" \
  "proves replay idempotency on the same deployment" \
  "writes current-run LF-001 evidence bound to run_id"; do
  if ! grep -qF "$sentinel" "$log"; then
    fail "EP-035-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all EP-035-owned LF-001 proofs observed"

total=$(grep -oE 'Tests  +[0-9]+ passed' "$log" | tail -1 | grep -oE '[0-9]+' | head -1)
ok "real one-package live-fire suite passed (${total} tests total)"

# Evidence freshness guard: current-run machine-readable evidence; stale
# evidence never satisfies.
if [ ! -f .agent/state/evidence/LF-001-ep035-m5.json ]; then
  fail "LF-001 evidence missing (LF-001-ep035-m5.json)"
fi
if ! grep -qE '"node": ?"EP-035"' .agent/state/evidence/LF-001-ep035-m5.json; then
  fail "LF-001 evidence not bound to EP-035"
fi
if ! grep -qE '"milestone": ?"M5"' .agent/state/evidence/LF-001-ep035-m5.json; then
  fail "LF-001 evidence not bound to M5"
fi
if ! grep -qE '"lf_id": ?"LF-001"' .agent/state/evidence/LF-001-ep035-m5.json; then
  fail "LF-001 evidence not bound to LF-001"
fi
if ! grep -qE '"artifact_hash": ?"[0-9a-f]{64}"' .agent/state/evidence/LF-001-ep035-m5.json; then
  fail "LF-001 evidence lacks a real artifact hash"
fi
if ! find .agent/state/evidence/LF-001-ep035-m5.json -mmin -10 | grep -q .; then
  fail "LF-001 evidence is stale (older than 10 minutes)"
fi
if grep -qiE 'password|token|secret|BEGIN [A-Z ]*PRIVATE KEY' .agent/state/evidence/LF-001-ep035-m5.json; then
  fail "LF-001 evidence contains secret-shaped content"
fi
ok "current-run evidence fresh, bound, and redacted"

# LF runner integrity: the live-fire script must call THIS real gate,
# never a dangling proof-runner / nexus-cli.
if ! grep -q 'sh scripts/ep035-m5-tests.sh' scripts/live-fire/LF-001.sh; then
  fail "scripts/live-fire/LF-001.sh does not call the real M5 gate"
fi
if grep -q 'proof-runner.sh' scripts/live-fire/LF-001.sh; then
  fail "scripts/live-fire/LF-001.sh still delegates to the dangling proof-runner"
fi
if grep -qE '(-p nexus-cli|proof run|nexusctl proof)' scripts/live-fire/LF-001.sh; then
  fail "scripts/live-fire/LF-001.sh still invokes the phantom proof runner"
fi
if grep -qE '(-p nexus-cli|proof run|nexusctl proof|proof-runner.sh)' scripts/ep035-one-package-build.sh; then
  fail "one-package bundle builder still invokes the phantom proof runner"
fi
ok "LF-001 wired to the real gate; phantom proof-runner/nexus-cli removed"

# M1-M4 regressions: M5 must not weaken the prior milestones, and LF-001
# must not substitute M3/M4 proofs for its own journey.
sh scripts/ep035-m1-tests.sh >>"$log" 2>&1 || fail "M1 regression failed" "$log"
ok "EP-035 M1 regression green"
sh scripts/ep035-m2-tests.sh >>"$log" 2>&1 || fail "M2 regression failed" "$log"
ok "EP-035 M2 regression green"
sh scripts/ep035-m3-tests.sh >>"$log" 2>&1 || fail "M3 regression failed" "$log"
ok "EP-035 M3 regression green"
sh scripts/ep035-m4-tests.sh >>"$log" 2>&1 || fail "M4 regression failed" "$log"
ok "EP-035 M4 regression green"

# Orphan guard: no leftover EP-035 containers, no stray processes, no
# scratch markers. (Ambient tooling LSP tsservers are NOT EP-035-owned
# processes and are excluded by construction.)
leftovers=$(docker ps -aq --filter name=^/nexus-ep035- | wc -l)
if [ "$leftovers" -ne 0 ]; then
  fail "leftover nexus-ep035-* containers: $leftovers"
fi
stray=$(ps aux | grep -E '[v]itest.*(livefire/deployment|onboarding)|[n]ode.*ep035_lf001' | sed '/^$/d')
if [ -n "$stray" ]; then
  echo "EP-035 M5 orphan guard: FAIL - stray processes:" >&2
  echo "$stray" >&2
  exit 1
fi
ok "orphan guard clean"

# Milestone artifact/fence checks.
for f in \
  .agent/milestone-files/EP-035-M5.txt \
  .agent/node-contracts/EP-035.md \
  "$PKG/package.json" \
  scripts/ep035-one-package-build.sh; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-035 M5: ok"
