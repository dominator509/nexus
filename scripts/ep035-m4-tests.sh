#!/usr/bin/env sh
# EP-035 M4 gate: forced failures, abuse cases, and observability.
#
# Runs the @nexus/onboarding-failure suite (tests/onboarding/) against
# REAL ephemeral PostgreSQL 18.4 + NATS 2.14.3 containers (digest-pinned
# per COMPONENT_REGISTRY.yaml) with vacuity guards, production-import
# guards, anti-masking sentinels, the ops diagnostic sanity check, and
# M1-M3 regressions.
#
# Vacuity guards are required: `vitest -t <filter>` exits 0 on a
# zero-match filter (EP-001 gate-masking class). The gate observes the
# exact ep035_failure_* test names and a real non-zero passing count.
set -eu
export CI=true

log="/tmp/ep035-m4-tests.log"
: > "$log"

fail() {
  echo "EP-035 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-035 M4 gate: $1"; }

PNPM="${PNPM_BIN:-/root/.local/share/mise/installs/pnpm/11.17.0/pnpm}"

# Vacuity guard 0: the failure package must exist.
if [ ! -f tests/onboarding/package.json ]; then
  fail "tests/onboarding/package.json missing"
fi
ok "failure package present"

# Vacuity guard 0b: all owned failure test files must exist.
for f in \
  src/__tests__/ep035_failure_unavailable_dependency.test.ts \
  src/__tests__/ep035_failure_timeout_budget.test.ts \
  src/__tests__/ep035_failure_malformed_input.test.ts \
  src/__tests__/ep035_failure_duplicate_request.test.ts \
  src/__tests__/ep035_failure_denied_permission.test.ts \
  src/__tests__/ep035_failure_partial_side_effect.test.ts \
  src/__tests__/ep035_failure_observability.test.ts \
  ops/onboarding-diag.sh; do
  if [ ! -f "tests/onboarding/$f" ]; then
    fail "tests/onboarding/$f missing"
  fi
done
ok "failure suite and ops diagnostic present"

# Production-import guard: the suite must import the REAL production
# integration layer and never substitute a mock-only clone.
if ! grep -q 'OwnerBootstrapStore' tests/onboarding/src/__tests__/ep035_failure_unavailable_dependency.test.ts; then
  fail "unavailable-dependency suite does not import production OwnerBootstrapStore"
fi
if ! grep -q 'EnrollmentTokenStore' tests/onboarding/src/__tests__/ep035_failure_denied_permission.test.ts; then
  fail "denied-permission suite does not import production EnrollmentTokenStore"
fi
if ! grep -q 'RecoveryCheckpointStore' tests/onboarding/src/__tests__/ep035_failure_partial_side_effect.test.ts; then
  fail "partial-side-effect suite does not import production RecoveryCheckpointStore"
fi
if ! grep -q 'OnboardingEventPublisher' tests/onboarding/src/__tests__/ep035_failure_observability.test.ts; then
  fail "observability suite does not import production OnboardingEventPublisher"
fi
if grep -rq 'vi.mock\|mock(' tests/onboarding/src/__tests__; then
  fail "mock-only substitute detected in failure suite"
fi
ok "production components imported; no mock-only substitute"

# Real typecheck.
if ! (cd tests/onboarding && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Real test run: the full failure suite through vitest, file-serial
# (each file spawns its own postgres+nats containers; parallel startup
# under load caused NATS subscriber timeouts - M3 lesson).
if ! (cd tests/onboarding && "$PNPM" exec vitest run src/__tests__ --no-file-parallelism >>"$log" 2>&1); then
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
# Vacuity guard 3: exact owned failure-file sentinels observed.
for name in \
  ep035_failure_unavailable_dependency \
  ep035_failure_timeout_budget \
  ep035_failure_malformed_input \
  ep035_failure_duplicate_request \
  ep035_failure_denied_permission \
  ep035_failure_partial_side_effect \
  ep035_failure_observability; do
  if ! grep -q "$name" "$log"; then
    fail "owned failure suite $name not observed" "$log"
  fi
done
ok "failure suite green (real containers, non-zero pass, zero fail/skip)"

# Guard: cleanup/orphan check - no leftover ep035 containers.
leftovers=$(docker ps -aq --filter name=^/nexus-ep035- | wc -l)
if [ "$leftovers" -ne 0 ]; then
  fail "leftover nexus-ep035-* containers: $leftovers"
fi
ok "no orphan containers"

# Ops diagnostic sanity: fails closed on unreachable providers.
diag_rc=0
sh tests/onboarding/ops/onboarding-diag.sh diagnose 1 1 >/dev/null 2>&1 || diag_rc=$?
if [ "$diag_rc" -ne 3 ]; then
  fail "ops diagnostic did not fail closed on unreachable providers (rc=$diag_rc)"
fi
ok "ops diagnostic fails closed on unreachable providers"

# M1/M2/M3 regressions.
sh scripts/ep035-m1-tests.sh >/dev/null 2>&1 || fail "M1 regression failed"
ok "EP-035 M1 regression green"
sh scripts/ep035-m2-tests.sh >/dev/null 2>&1 || fail "M2 regression failed"
ok "EP-035 M2 regression green"
sh scripts/ep035-m3-tests.sh >/dev/null 2>&1 || fail "M3 regression failed"
ok "EP-035 M3 regression green"

echo "EP-035 M4: ok"
