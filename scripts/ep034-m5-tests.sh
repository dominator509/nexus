#!/usr/bin/env sh
# EP-034 M5 gate: live-fire journeys (LF-004 multi-user-identity,
# LF-022 mobile-step-up) through the REAL flutter/dart machinery with
# vacuity guards, current-run evidence freshness, M1-M4 regressions,
# and orphan guard.
#
# M5 owns:
#   - LF-004: enroll two adults + one restricted user; prove separate
#     context, permissions, preferences, and mobile devices
#   - LF-022: request a high-risk action by voice (canonical AGENT
#     transcript seam), refuse voice-only authorization, approve via
#     the mobile step-up path, execute, and verify
#   - current-run machine-readable evidence under .agent/state/evidence/
#
# AUD-040: the native security surface is IMPLEMENTED at the channel
# contract - a real MethodChannel (nexus.mobile/security) with
# Android (androidx.biometric + AndroidKeyStore) and iOS
# (LocalAuthentication + Keychain) native implementations, and a
# Dart shell that FAILS CLOSED on unbound/error/malformed platform
# responses. HARDWARE verification (a physical biometric prompt on a
# device) is NOT ASSERTED - no device is present on CI - so the
# journeys compose REAL production components and the channel
# contract is proven over the real MethodChannel machinery, while
# hardware live-fire remains honestly unasserted.
set -eu
export CI=true

log="/tmp/ep034-m5-tests.log"
: > "$log"

fail() {
  echo "EP-034 M5 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-034 M5 gate: $1"; }

FLUTTER="${FLUTTER_BIN:-mise exec flutter -- flutter}"
DART="${DART_BIN:-mise exec flutter -- dart}"
PKG="tests/livefire/mobile"

# Vacuity guard 0: the live-fire package must exist.
if [ ! -f "$PKG/pubspec.yaml" ]; then
  fail "$PKG/pubspec.yaml missing"
fi

# Vacuity guard 0b: all owned live-fire source/test files must exist.
for f in \
  test/ep034_lf004_multi_user_test.dart \
  test/ep034_lf022_step_up_test.dart \
  test/ep034_lf_evidence_test.dart; do
  if [ ! -f "$PKG/$f" ]; then
    fail "$PKG/$f missing"
  fi
done
ok "live-fire package and owned source/test files present"

# Production-import guard: journeys compose REAL production components.
if ! grep -q 'package:nexus_mobile/nexus_mobile.dart' "$PKG/test/ep034_lf004_multi_user_test.dart"; then
  fail "LF-004 proof does not import production nexus_mobile contracts"
fi
if ! grep -q 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart' "$PKG/test/ep034_lf004_multi_user_test.dart"; then
  fail "LF-004 proof does not import production nexus_mobile_contracts behavior"
fi
if ! grep -q 'package:nexus_mobile_contracts/nexus_mobile_contracts.dart' "$PKG/test/ep034_lf022_step_up_test.dart"; then
  fail "LF-022 proof does not import production nexus_mobile_contracts behavior"
fi
ok "production components imported; no mock-only substitute"

# Real analyze: flutter analyze must pass (compile/typecheck gate).
if ! (cd "$PKG" && $FLUTTER analyze >>"$log" 2>&1); then
  fail "flutter analyze failed" "$log"
fi
ok "flutter analyze clean"

# Real format check: dart format must not report drift.
if ! (cd "$PKG" && $DART format --output=none --set-exit-if-changed lib test >>"$log" 2>&1); then
  fail "dart format drift (run dart format)" "$log"
fi
ok "dart format clean"

# Real test run: the full live-fire suite, sequential + expanded so
# every owned proof name is observable.
if ! (cd "$PKG" && $FLUTTER test --reporter expanded -j 1 >>"$log" 2>&1); then
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

# Vacuity guard 3 (anti-masking): EP-034-owned live-fire suites ran.
for sentinel in ep034_lf004_multi_user ep034_lf022_step_up ep034_lf_evidence; do
  if ! grep -q "$sentinel" "$log"; then
    fail "EP-034-owned live-fire suite did not run: $sentinel (anti-masking guard)" "$log"
  fi
done

# Vacuity guard 4: exact owned proof names observed.
for sentinel in \
  "enrolls two adults and one restricted user on distinct mobile devices" \
  "separate context: a prompt belongs to exactly one principal and device" \
  "separate permissions: restricted user cannot obtain high-risk approval" \
  "separate preferences: offline allowances are per-user, not shared" \
  "separate mobile devices: resolution records the acting device" \
  "voice-only authorization is refused (no device binding, AUTHORIZATION)" \
  "voice-only session strength never mints a high-risk approval by itself" \
  "mobile step-up approval executes and verifies" \
  "step-up telemetry carries correlation and never the prompt content" \
  "writes current-run LF-004 multi-user-identity evidence" \
  "writes current-run LF-022 mobile-step-up evidence"; do
  if ! grep -qF "$sentinel" "$log"; then
    fail "EP-034-owned proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all EP-034-owned live-fire proofs observed"

total=$(grep -oE '\+[1-9][0-9]*:' "$log" | tail -1 | tr -d '+:' | awk '{print $1}')
ok "real live-fire suite passed (${total} tests total)"

# Evidence freshness guard: current-run machine-readable evidence for
# both journeys; stale evidence never satisfies.
if [ ! -f .agent/state/evidence/LF-004-ep034-m5.json ]; then
  fail "LF-004 evidence missing (LF-004-ep034-m5.json)"
fi
if [ ! -f .agent/state/evidence/LF-022-ep034-m5.json ]; then
  fail "LF-022 evidence missing (LF-022-ep034-m5.json)"
fi
if ! grep -qE '"node": ?"EP-034"' .agent/state/evidence/LF-004-ep034-m5.json; then
  fail "LF-004 evidence not bound to EP-034"
fi
if ! grep -qE '"milestone": ?"M5"' .agent/state/evidence/LF-004-ep034-m5.json; then
  fail "LF-004 evidence not bound to M5"
fi
if ! grep -qE '"node": ?"EP-034"' .agent/state/evidence/LF-022-ep034-m5.json; then
  fail "LF-022 evidence not bound to EP-034"
fi
if ! grep -qE '"milestone": ?"M5"' .agent/state/evidence/LF-022-ep034-m5.json; then
  fail "LF-022 evidence not bound to M5"
fi
if ! find .agent/state/evidence/LF-004-ep034-m5.json -mmin -10 | grep -q .; then
  fail "LF-004 evidence is stale (older than 10 minutes)"
fi
if ! find .agent/state/evidence/LF-022-ep034-m5.json -mmin -10 | grep -q .; then
  fail "LF-022 evidence is stale (older than 10 minutes)"
fi
ok "current-run evidence fresh and bound (LF-004 + LF-022)"

# LF runner integrity: the live-fire scripts must call THIS real gate,
# never a dangling proof-runner / nexus-cli.
for lf in LF-004 LF-022; do
  if ! grep -q 'sh scripts/ep034-m5-tests.sh' "scripts/live-fire/$lf.sh"; then
    fail "scripts/live-fire/$lf.sh does not call the real M5 gate"
  fi
  if grep -q 'proof-runner.sh' "scripts/live-fire/$lf.sh"; then
    fail "scripts/live-fire/$lf.sh still delegates to the dangling proof-runner"
  fi
done
ok "LF-004/LF-022 live-fire wired to the real gate"

# M1-M4 regressions: M5 must not weaken the prior milestones.
if ! sh scripts/ep034-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression (mobile contract suite) failed" "$log"
fi
if ! sh scripts/ep034-m2-tests.sh >>"$log" 2>&1; then
  fail "M2 regression (behavior suite) failed" "$log"
fi
if ! sh scripts/ep034-m3-tests.sh >>"$log" 2>&1; then
  fail "M3 regression (e2e transport suite) failed" "$log"
fi
if ! sh scripts/ep034-m4-tests.sh >>"$log" 2>&1; then
  fail "M4 regression (failure suite) failed" "$log"
fi
ok "M1/M2/M3/M4 regressions green"

# Orphan guard: no stray flutter test workers or scratch markers.
stray=$(ps aux | grep -E '[f]lutter_tester|[d]art.*ep034' | sed '/^$/d')
if [ -n "$stray" ]; then
  echo "EP-034 M5 orphan guard: FAIL - stray processes:" >&2
  echo "$stray" >&2
  exit 1
fi
if [ -f /tmp/ep034-m5-scratch ]; then
  echo "EP-034 M5 orphan guard: FAIL - scratch marker present" >&2
  exit 1
fi
ok "orphan guard clean"

# Milestone artifact/fence checks.
for f in .agent/milestone-files/EP-034-M5.txt .agent/node-contracts/EP-034.md \
         .agent/execplans/EP-034-ios-and-android-mobile.md "$PKG/pubspec.yaml"; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-034 M5: ok"
