#!/usr/bin/env sh
# EP-033 M4 gate: run the @nexus/web-e2e forced-failure suite through
# the REAL pnpm/vitest machinery with vacuity guards, production-import
# guards, anti-masking sentinels, and M1-M3 regressions.
#
# The M4 changed-file fence is tests/e2e/web/ (plus the node script,
# gate script, and plan files), so the authoritative gate is the e2e
# failure package (tsc --noEmit + vitest ep033_failure_*). Vacuity
# guards are required: `vitest -t <filter>` exits 0 on a zero-match
# filter (EP-001 gate-masking class), and a green M4 must observe a
# real non-zero passing count, the exact EP-033-owned failure test
# names, zero skipped tests, and zero mocked substitutes.
set -eu
export CI=true

log="/tmp/ep033-m4-tests.log"
: > "$log"

fail() {
  echo "EP-033 M4 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-033 M4 gate: $1"; }

PNPM="${PNPM_BIN:-/root/.local/share/mise/installs/pnpm/11.17.0/pnpm}"

# Vacuity guard 0: the e2e failure package must exist.
if [ ! -f tests/e2e/web/package.json ]; then
  fail "tests/e2e/web/package.json missing"
fi

# Vacuity guard 0b: all four owned failure test files must exist.
for f in \
  src/__tests__/ep033_failure_malformed_input.test.ts \
  src/__tests__/ep033_failure_denied_policy.test.ts \
  src/__tests__/ep033_failure_state_authority.test.ts \
  src/__tests__/ep033_failure_content_prefs_telemetry.test.ts; do
  if [ ! -f "tests/e2e/web/$f" ]; then
    fail "tests/e2e/web/$f missing"
  fi
done
ok "e2e failure package and four failure files present"

# Production-import guard: the suite must import the REAL production
# components, never a mock-only substitute.
if ! grep -q 'DesktopCommandDispatcher' tests/e2e/web/src/__tests__/ep033_failure_denied_policy.test.ts; then
  fail "denied-policy suite does not import production DesktopCommandDispatcher"
fi
if ! grep -q 'DesktopCommandDispatcher' tests/e2e/web/src/__tests__/ep033_failure_state_authority.test.ts; then
  fail "state-authority suite does not import production DesktopCommandDispatcher"
fi
if ! grep -q 'DesktopShellRuntime' tests/e2e/web/src/__tests__/ep033_failure_state_authority.test.ts; then
  fail "state-authority suite does not use production DesktopShellRuntime"
fi
if ! grep -q 'DesktopApprovalFlow' tests/e2e/web/src/__tests__/ep033_failure_state_authority.test.ts; then
  fail "state-authority suite does not use production DesktopApprovalFlow"
fi
if ! grep -q 'DesktopTelemetry' tests/e2e/web/src/__tests__/ep033_failure_content_prefs_telemetry.test.ts; then
  fail "telemetry suite does not use production DesktopTelemetry"
fi
if grep -rqE 'vi\.mock\(|mock\([^)]*\)' tests/e2e/web/src; then
  fail "mock-only substitute detected in e2e failure suite"
fi
ok "production components imported; no mock-only substitute"

# Real typecheck: tsc --noEmit must pass (compile/typecheck gate).
if ! (cd tests/e2e/web && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Real test run: the full e2e failure suite through vitest. The
# verbose reporter prints each owned test name so the anti-masking
# greps can observe the exact proofs.
if ! (cd tests/e2e/web && "$PNPM" exec vitest run src/__tests__ --reporter=verbose >>"$log" 2>&1); then
  fail "vitest run failed" "$log"
fi

# vitest emits ANSI color codes even under CI=true; strip them so the
# vacuity greps observe plain text.
sed -i 's/\x1b\[[0-9;]*m//g' "$log"

# Vacuity guard 1: a non-zero number of tests actually ran.
if ! grep -qE 'Tests[[:space:]]+[1-9][0-9]* passed' "$log"; then
  fail "no tests ran (vacuity guard)" "$log"
fi

# Vacuity guard 2: zero failures observed.
if grep -qE '[1-9][0-9]* failed' "$log"; then
  fail "observed failed tests (vacuity guard)" "$log"
fi

# Vacuity guard 3: zero skipped/ignored tests.
if grep -qE 'Tests[[:space:]]+[0-9]+ passed \([0-9]+ skipped' "$log"; then
  fail "required tests were skipped (vacuity guard)" "$log"
fi

# Vacuity guard 4 (anti-masking): the exact EP-033-owned failure suites
# must be observed, proving the real e2e package ran - not the web
# unit suite, the desktop unit suite, or a zero-match filter.
for sentinel in \
  "ep033_failure_malformed_input.test.ts" \
  "ep033_failure_denied_policy.test.ts" \
  "ep033_failure_state_authority.test.ts" \
  "ep033_failure_content_prefs_telemetry.test.ts"; do
  if ! grep -q "$sentinel" "$log"; then
    fail "e2e failure file did not run: $sentinel (anti-masking guard)" "$log"
  fi
done

# Vacuity guard 5 (anti-masking): exact owned test names observed,
# proving the concrete failure behaviors executed.
for sentinel in \
  "rejects a session with unknown fields (deny-unknown fail closed)" \
  "refuses an R3 command without human approval (policy denied)" \
  "refuses an R4 command under POLICY approval (never auto-executes)" \
  "refuses dispatch under a revoked session (authorization denied)" \
  "refuses dispatch under an expired session and never replays" \
  "invalidates a business-A projection after switching to business B (stale handle)" \
  "refuses a consequential action while the backend is unavailable" \
  "refuses a consequential action while offline" \
  "never presents an offline payload as live: stale is not actionable" \
  "executes a duplicate request exactly once (idempotency ring)" \
  "rejects a reused idempotency key for a different request (conflict, no dispatch)" \
  "four-eyes abuse: one principal clicking twice never satisfies FOUR_EYES" \
  "rejects an approval-class downgrade payload (fabricated class)" \
  "hostile text inside command arguments cannot mint authority (R4 still denied)" \
  "treats hostile command-like text as inert message data" \
  "serialized preference blobs cannot overwrite security context" \
  "the persistence boundary refuses token-like values by content" \
  "desktop telemetry strips token-like canaries from every field" \
  "desktop telemetry strips secret-key and api-key canaries" \
  "the redacted logger never records private body content"; do
  if ! grep -qF "$sentinel" "$log"; then
    fail "EP-033-owned failure proof did not run: $sentinel (anti-masking guard)" "$log"
  fi
done
ok "all four e2e failure files and 20 owned proof names observed"

total=$(grep -oE 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | awk '{s+=$2} END {print s}')
ok "real e2e failure suite passed (${total} tests total)"

# M1-M3 regressions: M4 must not weaken the prior milestones.
if ! sh scripts/ep033-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression (web contract suite) failed" "$log"
fi
if ! sh scripts/ep033-m2-tests.sh >>"$log" 2>&1; then
  fail "M2 regression (desktop suite) failed" "$log"
fi
if ! sh scripts/ep033-m3-tests.sh >>"$log" 2>&1; then
  fail "M3 regression (ui suite) failed" "$log"
fi
ok "M1/M2/M3 regressions green"

# Orphan guard: the e2e suite starts no servers, but any stray vitest
# worker or scratch file would be an orphan. The bracket trick avoids
# matching this script's own grep.
stray=$(ps aux | grep -E '[v]itest.*tests/e2e/web' | sed '/^$/d')
if [ -n "$stray" ]; then
  echo "EP-033 M4 orphan guard: FAIL - stray vitest processes:" >&2
  echo "$stray" >&2
  exit 1
fi
if [ -f /tmp/ep033-m4-scratch ]; then
  echo "EP-033 M4 orphan guard: FAIL - scratch marker present" >&2
  exit 1
fi
ok "orphan guard clean"

# Milestone artifact/fence checks.
for f in .agent/milestone-files/EP-033-M4.txt .agent/node-contracts/EP-033.md \
         .agent/execplans/EP-033-web-dashboard-and-desktop.md tests/e2e/web/package.json; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-033 M4: ok"
