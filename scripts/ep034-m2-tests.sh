#!/usr/bin/env sh
# EP-034 M2 gate: run the @nexus_mobile_contracts core-behavior suite
# through the REAL flutter/dart machinery with vacuity guards.
#
# The M2 changed-file fence is packages/mobile-contracts/ plus the
# node script and plan files, so the authoritative gate is the
# mobile core-behavior suite (flutter analyze + flutter test
# ep034_unit_*) plus a dependency-direction proof. Vacuity guards are
# required: `flutter test --plain-name <filter>` exits 0 on a
# zero-match filter (EP-001 gate-masking class), so a green M2 must
# observe a real non-zero passing count, EP-034-owned test names, and
# zero skipped/filtered tests.
set -eu
export CI=true

log="/tmp/ep034-m2-tests.log"
: > "$log"

fail() {
  echo "EP-034 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-034 M2 gate: $1"; }

FLUTTER="${FLUTTER_BIN:-mise exec flutter -- flutter}"
DART="${DART_BIN:-mise exec flutter -- dart}"

# Vacuity guard 0: the M2 package must exist.
if [ ! -f packages/mobile-contracts/pubspec.yaml ]; then
  fail "packages/mobile-contracts/pubspec.yaml missing"
fi

# Vacuity guard 0b: the owned production behavior sources must exist.
for f in \
  lib/nexus_mobile_contracts.dart \
  lib/src/behavior/approval_binding.dart \
  lib/src/behavior/offline_policy.dart \
  lib/src/behavior/telemetry.dart; do
  if [ ! -f "packages/mobile-contracts/$f" ]; then
    fail "packages/mobile-contracts/$f missing"
  fi
done
ok "mobile behavior package and sources present"

# Real analyze: flutter analyze must pass (compile/typecheck gate).
if ! (cd packages/mobile-contracts && $FLUTTER analyze >>"$log" 2>&1); then
  fail "flutter analyze failed" "$log"
fi
ok "flutter analyze clean"

# Real format check: dart format must not report drift.
if ! (cd packages/mobile-contracts && $DART format --output=none --set-exit-if-changed lib test >>"$log" 2>&1); then
  fail "dart format drift (run dart format)" "$log"
fi
ok "dart format clean"

# Real test run: the full ep034_unit suite through flutter test.
if ! (cd packages/mobile-contracts && $FLUTTER test >>"$log" 2>&1); then
  fail "flutter test failed" "$log"
fi

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE '\+[1-9][0-9]*:' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures observed.
if grep -qE '\-[1-9][0-9]*:' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3 (anti-masking): the dependency-direction test ran.
if ! grep -q 'ep034_unit_dependency_direction' "$log"; then
  fail "dependency-direction test did not run" "$log"
fi

# Vacuity guard 4 (anti-masking): EP-034-owned behavior suites observed,
# proving the real M2 suite ran - not a prior milestone's tests.
for sentinel in ep034_unit_approval_binding ep034_unit_offline_policy \
  ep034_unit_telemetry ep034_unit_dependency_direction; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-034-owned behavior suite did not run: $sentinel (anti-masking guard)" "$log"
  fi
done

# Vacuity guard 5: exact owned proof names observed.
for sentinel in \
  "high-risk approval binds to device: wrong acting device is AUTHORIZATION" \
  "high-risk approval binds to user: wrong acting principal is AUTHORIZATION" \
  "revoked device binding refuses high-risk approval" \
  "expired approval prompt is not actionable (POLICY)" \
  "R3 approval with POLICY class is refused (POLICY)" \
  "R4 approval with HUMAN class is accepted" \
  "approval resolution is idempotent: duplicate approve returns the same resolution" \
  "approval resolved once cannot be re-denied (CONFLICT)" \
  "denied approval cannot be re-approved (CONFLICT)" \
  "offline low-risk control with fresh cached allowance is allowed" \
  "stale cached allowance is denied (POLICY)" \
  "R4 control never runs from cached policy (POLICY)" \
  "cache entry cannot upgrade requested risk class" \
  "CachedPolicyEntry rejects unknown fields (VOCABULARY)" \
  "telemetry redaction strips bearer-shaped canary" \
  "telemetry never emits raw prompt content" \
  "behavior sources import only the nexus_mobile contract barrel"; do
  if ! grep -qF "$sentinel" "$log"; then
    fail "EP-034-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all EP-034-owned behavior proofs observed"

total=$(grep -oE '\+[1-9][0-9]*:' "$log" | tail -1 | tr -d '+:' | awk '{print $1}')
ok "real mobile behavior suite passed (${total} tests total)"

# Milestone artifact/fence checks: M2 fence paths exist.
for f in .agent/milestone-files/EP-034-M2.txt .agent/node-contracts/EP-034.md \
         .agent/execplans/EP-034-ios-and-android-mobile.md packages/mobile-contracts/pubspec.yaml; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-034 M2: ok"
