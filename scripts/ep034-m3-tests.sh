#!/usr/bin/env sh
# EP-034 M3 gate: run the @nexus_mobile_e2e integration suite through
# the REAL flutter/dart machinery with vacuity guards.
#
# The M3 changed-file fence is tests/e2e/mobile/ plus the node script
# and plan files, so the authoritative gate is the integration suite
# (flutter analyze + flutter test ep034_integration_*) which proves
# contract behavior across a REAL dart:io HTTP transport boundary.
set -eu
export CI=true

log="/tmp/ep034-m3-tests.log"
: > "$log"

fail() {
  echo "EP-034 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-034 M3 gate: $1"; }

FLUTTER="${FLUTTER_BIN:-mise exec flutter -- flutter}"
DART="${DART_BIN:-mise exec flutter -- dart}"

# Vacuity guard 0: the e2e package must exist.
if [ ! -f tests/e2e/mobile/pubspec.yaml ]; then
  fail "tests/e2e/mobile/pubspec.yaml missing"
fi

# Vacuity guard 0b: the owned integration sources must exist.
for f in \
  test/ep034_integration_transport_test.dart \
  test/ep034_integration_support.dart; do
  if [ ! -f "tests/e2e/mobile/$f" ]; then
    fail "tests/e2e/mobile/$f missing"
  fi
done
ok "mobile e2e package and sources present"

# Real analyze: flutter analyze must pass (compile/typecheck gate).
if ! (cd tests/e2e/mobile && $FLUTTER analyze >>"$log" 2>&1); then
  fail "flutter analyze failed" "$log"
fi
ok "flutter analyze clean"

# Real format check: dart format must not report drift.
if ! (cd tests/e2e/mobile && $DART format --output=none --set-exit-if-changed lib test >>"$log" 2>&1); then
  fail "dart format drift (run dart format)" "$log"
fi
ok "dart format clean"

# Real test run: the full ep034_integration suite through flutter test.
if ! (cd tests/e2e/mobile && $FLUTTER test --reporter expanded -j 1 >>"$log" 2>&1); then
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

# Vacuity guard 3 (anti-masking): EP-034-owned integration suite ran.
if ! grep -q 'ep034_integration_transport' "$log"; then
  fail "integration suite did not run (anti-masking guard)" "$log"
fi

# Vacuity guard 4: exact owned proof names observed.
for sentinel in \
  "readiness: server health endpoint reports ok over real HTTP" \
  "approval prompt round-trips canonical JSON over real transport" \
  "idempotent retry over transport resolves exactly once" \
  "divergent retry with same idempotency key is CONFLICT" \
  "slow server response exceeds client timeout (TIMEOUT at transport)" \
  "client cancellation reaches the server as an aborted request" \
  "typed SPEC-006 error crosses transport with correlation preserved" \
  "server audit records correlation across the boundary" \
  "cleanup: server close releases the port for rebinding" \
  "transport unavailable fails closed (connection refused)"; do
  if ! grep -qF "$sentinel" "$log"; then
    fail "EP-034-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all EP-034-owned integration proofs observed"

total=$(grep -oE '\+[1-9][0-9]*:' "$log" | tail -1 | tr -d '+:' | awk '{print $1}')
ok "real mobile integration suite passed (${total} tests total)"

# Milestone artifact/fence checks: M3 fence paths exist.
for f in .agent/milestone-files/EP-034-M3.txt .agent/node-contracts/EP-034.md \
         .agent/execplans/EP-034-ios-and-android-mobile.md tests/e2e/mobile/pubspec.yaml; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-034 M3: ok"
