#!/usr/bin/env sh
# EP-034 M4 gate: run the @nexus_mobile_failure suite through the REAL
# flutter/dart machinery with vacuity guards.
#
# The M4 changed-file fence is tests/accessibility/mobile/ (machine
# fence path; content is the forced-failure suite per ExecPlan) plus
# the node script and plan files. The authoritative gate is the
# failure suite (flutter analyze + flutter test ep034_failure_*)
# proving fail-closed behavior over REAL production components with
# real failure mechanisms.
set -eu
export CI=true

log="/tmp/ep034-m4-tests.log"
: > "$log"

fail() {
  echo "EP-034 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-034 M4 gate: $1"; }

FLUTTER="${FLUTTER_BIN:-mise exec flutter -- flutter}"
DART="${DART_BIN:-mise exec flutter -- dart}"

# Vacuity guard 0: the M4 package must exist.
if [ ! -f tests/accessibility/mobile/pubspec.yaml ]; then
  fail "tests/accessibility/mobile/pubspec.yaml missing"
fi

# Vacuity guard 0b: the owned failure sources must exist.
for f in \
  test/ep034_failure_malformed_test.dart \
  test/ep034_failure_authority_test.dart \
  test/ep034_failure_idempotency_test.dart \
  test/ep034_failure_observability_test.dart \
  test/ep034_failure_transport_test.dart \
  test/ep034_failure_support.dart; do
  if [ ! -f "tests/accessibility/mobile/$f" ]; then
    fail "tests/accessibility/mobile/$f missing"
  fi
done
ok "mobile failure package and sources present"

# Real analyze: flutter analyze must pass (compile/typecheck gate).
if ! (cd tests/accessibility/mobile && $FLUTTER analyze >>"$log" 2>&1); then
  fail "flutter analyze failed" "$log"
fi
ok "flutter analyze clean"

# Real format check: dart format must not report drift (test-only
# package: no lib/ directory).
if ! (cd tests/accessibility/mobile && $DART format --output=none --set-exit-if-changed test >>"$log" 2>&1); then
  fail "dart format drift (run dart format)" "$log"
fi
ok "dart format clean"

# Real test run: the full ep034_failure suite through flutter test.
# -j 1 (sequential) + expanded reporter so every owned test name is
# observable in the log; parallel runs interleave labels and hide
# fast synchronous suite names (EP-033 worker-starvation lesson).
if ! (cd tests/accessibility/mobile && $FLUTTER test --reporter expanded -j 1 >>"$log" 2>&1); then
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

# Vacuity guard 3 (anti-masking): EP-034-owned failure suites ran.
for sentinel in ep034_failure_malformed ep034_failure_authority \
  ep034_failure_idempotency ep034_failure_observability \
  ep034_failure_transport; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-034-owned failure suite did not run: $sentinel (anti-masking guard)" "$log"
  fi
done

# Vacuity guard 4: exact owned proof names observed.
for sentinel in \
  "unknown field in approval wire input is rejected with VOCABULARY" \
  "fabricated approval class enum is rejected with VOCABULARY" \
  "missing required value is a VALIDATION failure" \
  "invalid identifier (bad uuid) is a VALIDATION failure" \
  "unknown problem-details code is rejected with VOCABULARY" \
  "fabricated session field is rejected with VOCABULARY" \
  "fabricated device trust level is rejected with VOCABULARY" \
  "wrong acting device cannot resolve the approval (AUTHORIZATION)" \
  "wrong acting principal cannot resolve the approval (AUTHORIZATION)" \
  "revoked device binding is terminal for approval (AUTHORIZATION)" \
  "revoked session cannot authorize a consequential action" \
  "R4 approval with POLICY class never mints authority (POLICY)" \
  "expired approval is never actionable (POLICY)" \
  "offline R3 control is denied from cached policy (POLICY)" \
  "stale cached allowance is never actionable (POLICY)" \
  "unknown capability fails closed offline (POLICY)" \
  "duplicate approval resolution executes exactly once" \
  "divergent re-resolution of a resolved approval is CONFLICT" \
  "double-deny is idempotent; approve after deny is CONFLICT" \
  "partial side effect: timed-out resolve retried with same key does not double-execute" \
  "corrupted resolution payload over transport is 422 VOCABULARY" \
  "bearer-shaped canary never leaves in telemetry" \
  "token-shaped canary never leaves in telemetry" \
  "private prompt content never appears in telemetry" \
  "correlation and outcome remain observable after redaction" \
  "transport unavailable fails closed with SocketException" \
  "slow server response exceeds client timeout" \
  "client cancellation aborts the in-flight request server-side" \
  "unknown route fails closed with NOT_FOUND"; do
  if ! grep -qF "$sentinel" "$log"; then
    fail "EP-034-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all EP-034-owned failure proofs observed"

total=$(grep -oE '\+[1-9][0-9]*:' "$log" | tail -1 | tr -d '+:' | awk '{print $1}')
ok "real mobile failure suite passed (${total} tests total)"

# Milestone artifact/fence checks: M4 fence paths exist.
for f in .agent/milestone-files/EP-034-M4.txt .agent/node-contracts/EP-034.md \
         .agent/execplans/EP-034-ios-and-android-mobile.md tests/accessibility/mobile/pubspec.yaml; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-034 M4: ok"
