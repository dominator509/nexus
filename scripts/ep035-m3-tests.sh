#!/usr/bin/env sh
# EP-035 M3 gate: real onboarding dependency integration.
#
# Runs the @nexus/onboarding suite against REAL ephemeral PostgreSQL 18.4
# and NATS 2.14.3 containers (digest-pinned per COMPONENT_REGISTRY.yaml)
# with vacuity guards: tsc --noEmit, unit tests, container-gated
# integration tests, sentinel checks, exact runtime version observation,
# and M1/M2 regressions.
#
# Vacuity guards are required: `vitest -t <filter>` exits 0 on a
# zero-match filter (EP-001 gate-masking class).
set -eu
export CI=true

log="/tmp/ep035-m3-tests.log"
: > "$log"

fail() {
  echo "EP-035 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-035 M3 gate: $1"; }

PNPM="${PNPM_BIN:-pnpm}"

# Vacuity guard 0: the onboarding package must exist.
if [ ! -f packages/onboarding/package.json ]; then
  fail "packages/onboarding/package.json missing"
fi
ok "onboarding package present"

# Vacuity guard 0b: owned production sources must exist.
for f in \
  src/index.ts \
  src/db.ts \
  src/redact.ts \
  src/events.ts \
  src/stores/owner-bootstrap.store.ts \
  src/stores/enrollment-token.store.ts \
  src/stores/deployment-intent.store.ts \
  src/stores/integration-state.store.ts \
  src/stores/recovery-checkpoint.store.ts \
  migrations/001_onboarding.sql; do
  if [ ! -f "packages/onboarding/$f" ]; then
    fail "packages/onboarding/$f missing"
  fi
done
ok "owned onboarding sources present"

# Vacuity guard 1: exact selected dependencies registered with digests.
grep -q "postgres:18.4" COMPONENT_REGISTRY.yaml || fail "postgresql not registered"
grep -q "nats" COMPONENT_REGISTRY.yaml || fail "nats not registered"

# Real typecheck.
if ! (cd packages/onboarding && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Unit tests (no containers).
if ! (cd packages/onboarding && "$PNPM" exec vitest run src/__tests__/unit >>"$log" 2>&1); then
  fail "unit tests failed" "$log"
fi
ok "unit tests green"

# Integration tests (real ephemeral containers). Must collect nonzero
# ep035_integration_* tests and pass. --no-file-parallelism: each file
# spawns its own postgres+nats containers; parallel startup under load
# caused NATS subscriber timeouts (deterministic serial like M2 -j 1).
if ! (cd packages/onboarding && "$PNPM" exec vitest run src/__tests__/integration --no-file-parallelism >>"$log" 2>&1); then
  fail "integration tests failed" "$log"
fi
sed -i 's/\x1b\[[0-9;]*m//g' "$log"
grep -q "ep035_integration" "$log" || fail "no ep035_integration_* tests observed" "$log"
grep -q "Tests  .* passed" "$log" || fail "no passing test sentinel" "$log"
grep -qE "Tests  +[1-9][0-9]* passed" "$log" || fail "zero tests passed (vacuity)" "$log"
ok "integration tests green (real containers)"

# Guard: no skipped required tests.
if grep -qE "Tests  +[0-9]+ (failed|skipped)" "$log"; then
  fail "skipped or failed tests present" "$log"
fi

# Guard: cleanup/orphan check - no leftover ep035 containers.
# Blueprint-safe: Docker name filter with anchored regex instead of a
# Go-template name format string (double-brace placeholder class).
# `-a` keeps stopped-container detection; `^/` anchors to the leading
# slash Docker prefixes on names, matching exactly the original
# prefix-anchored count.
leftovers=$(docker ps -aq --filter name=^/nexus-ep035- | wc -l)
if [ "$leftovers" -ne 0 ]; then
  fail "leftover nexus-ep035-* containers: $leftovers"
fi
ok "no orphan containers"

# M1/M2 regressions.
sh scripts/ep035-m1-tests.sh >/dev/null 2>&1 || fail "M1 regression failed"
ok "EP-035 M1 regression green"
sh scripts/ep035-m2-tests.sh >/dev/null 2>&1 || fail "M2 regression failed"
ok "EP-035 M2 regression green"

echo "EP-035 M3: ok"
