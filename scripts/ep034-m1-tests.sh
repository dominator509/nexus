#!/usr/bin/env sh
# EP-034 M1 gate: run the @nexus_mobile Flutter contract suite through
# the REAL flutter/dart machinery with vacuity guards.
#
# The M1 changed-file fence is apps/mobile/ (contract package) plus
# the node script and plan files, so the authoritative gate is the
# mobile contract suite (flutter analyze + flutter test ep034_unit_*)
# plus a dependency-direction proof. Vacuity guards are required:
# `flutter test --plain-name <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class), so a green M1 must observe a real
# non-zero passing count, the dependency-direction test, EP-034-owned
# test names, and zero skipped/filtered tests.
set -eu
export CI=true

log="/tmp/ep034-m1-tests.log"
: > "$log"

fail() {
  echo "EP-034 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-034 M1 gate: $1"; }

FLUTTER="${FLUTTER_BIN:-mise exec flutter -- flutter}"
DART="${DART_BIN:-mise exec flutter -- dart}"

# Vacuity guard 0: the mobile package must exist.
if [ ! -f apps/mobile/pubspec.yaml ]; then
  fail "apps/mobile/pubspec.yaml missing"
fi

# Vacuity guard 0b: the owned production contract sources must exist.
for f in \
  lib/nexus_mobile.dart \
  lib/src/contracts/errors.dart \
  lib/src/contracts/validate.dart \
  lib/src/contracts/device.dart \
  lib/src/contracts/session.dart \
  lib/src/contracts/approvals.dart \
  lib/src/contracts/enrollment.dart \
  lib/src/contracts/voice.dart \
  lib/src/contracts/bluetooth.dart \
  lib/src/contracts/secure_store.dart \
  lib/src/contracts/push.dart \
  lib/src/contracts/remote.dart; do
  if [ ! -f "apps/mobile/$f" ]; then
    fail "apps/mobile/$f missing"
  fi
done
ok "mobile contract package and sources present"

# Real analyze: flutter analyze must pass (compile/typecheck gate).
if ! (cd apps/mobile && $FLUTTER analyze >>"$log" 2>&1); then
  fail "flutter analyze failed" "$log"
fi
ok "flutter analyze clean"

# Real format check: dart format must not report drift.
if ! (cd apps/mobile && $DART format --output=none --set-exit-if-changed lib test >>"$log" 2>&1); then
  fail "dart format drift (run dart format)" "$log"
fi
ok "dart format clean"

# Real test run: the full ep034_unit suite through flutter test.
if ! (cd apps/mobile && $FLUTTER test --reporter expanded -j 1 >>"$log" 2>&1); then
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

# Vacuity guard 4 (anti-masking): EP-034-owned contract tests observed,
# proving the real mobile suite ran - not a prior node's tests.
for sentinel in ep034_unit_validation ep034_unit_serialization \
  ep034_unit_schema_parity ep034_unit_surfaces; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-034-owned contract test did not run: $sentinel (anti-masking guard)" "$log"
  fi
done

# Vacuity guard 5: exact owned proof names observed.
for sentinel in \
  "deny-unknown rejects a fabricated session field" \
  "unknown enum value is rejected with VOCABULARY never defaulted" \
  "fabricated approval class cannot mint authority" \
  "undersized idempotency key is a validation failure" \
  "session expiry is terminal for usability" \
  "contract layer exposes the eight EP-034 public interfaces" \
  "MobileSession round-trips wire JSON" \
  "ApprovalPrompt round-trips wire JSON with full disclosure" \
  "device-identity schema kind enum matches DeviceKind" \
  "auth-session schema grant_flow matches GrantFlow"; do
  if ! grep -qF "$sentinel" "$log"; then
    fail "EP-034-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all EP-034-owned contract proofs observed"

total=$(grep -oE '\+[1-9][0-9]*:' "$log" | tail -1 | tr -d '+:' | awk '{print $1}')
ok "real mobile contract suite passed (${total} tests total)"

# Milestone artifact/fence checks: M1 fence paths exist.
for f in .agent/milestone-files/EP-034-M1.txt .agent/node-contracts/EP-034.md \
         .agent/execplans/EP-034-ios-and-android-mobile.md apps/mobile/pubspec.yaml; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-034 M1: ok"
