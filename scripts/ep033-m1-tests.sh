#!/usr/bin/env sh
# EP-033 M1 gate: run the @nexus/web dashboard contract suite through
# the REAL pnpm/vitest machinery with vacuity guards.
#
# The M1 changed-file fence is apps/web/ (contract package) plus the
# node script and plan files, so the authoritative gate is the web
# contract suite (tsc --noEmit + vitest ep033_unit_*) plus a
# dependency-direction proof. Vacuity guards are required: `vitest -t
# <filter>` exits 0 on a zero-match filter (EP-001 gate-masking
# class), so a green M1 must observe a real non-zero passing count,
# the dependency-direction test, an EP-033-owned test name, and zero
# skipped/filtered tests.
set -eu
export CI=true

log="/tmp/ep033-m1-tests.log"
: > "$log"

fail() {
  echo "EP-033 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-033 M1 gate: $1"; }

PNPM="${PNPM_BIN:-/root/.local/share/mise/installs/pnpm/11.17.0/pnpm}"

# Vacuity guard 0: the web package must exist.
if [ ! -f apps/web/package.json ]; then
  fail "apps/web/package.json missing"
fi

# Vacuity guard 0b: the owned production contract sources must exist.
for f in \
  src/index.ts \
  src/contracts/errors.ts \
  src/contracts/validate.ts \
  src/contracts/session.ts \
  src/contracts/state.ts \
  src/contracts/capability.ts \
  src/contracts/command.ts \
  src/contracts/approval-center.ts \
  src/contracts/events.ts \
  src/contracts/preferences.ts \
  src/contracts/accessibility.ts \
  src/contracts/logging.ts \
  src/contracts/dashboard-shell.ts \
  src/contracts/chat-workspace.ts \
  src/contracts/objective-view.ts \
  src/contracts/fleet-view.ts \
  src/contracts/security-console.ts \
  src/contracts/provider-settings.ts \
  src/contracts/audit-explorer.ts; do
  if [ ! -f "apps/web/$f" ]; then
    fail "apps/web/$f missing"
  fi
done
ok "web contract package and sources present"

# Real typecheck: tsc --noEmit must pass (compile/typecheck gate).
if ! (cd apps/web && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Real test run: the full ep033_unit suite through vitest.
if ! (cd apps/web && "$PNPM" exec vitest run src/__tests__ >>"$log" 2>&1); then
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

# Vacuity guard 3: zero skipped/ignored tests (no required test may be
# skipped or filtered out).
if grep -qE 'Tests[[:space:]]+[0-9]+ passed \([0-9]+ skipped' "$log"; then
  fail "required tests were skipped (vacuity guard)" "$log"
fi

# Vacuity guard 4: the dependency-direction test ran and passed.
if ! grep -q 'ep033_unit_dependency_direction' "$log"; then
  fail "dependency-direction test did not run" "$log"
fi

# Vacuity guard 5 (anti-masking): an EP-033-owned dashboard contract
# test must be observed. This fails if the gate accidentally executes
# only a prior node's tests.
if ! grep -q 'ep033_unit_session' "$log"; then
  fail "EP-033-owned session contract test did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep033_unit_capability' "$log"; then
  fail "EP-033-owned capability contract test did not run (anti-masking guard)" "$log"
fi

total=$(grep -oE 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | awk '{s+=$2} END {print s}')
ok "real suite passed (${total} tests total)"

# Milestone artifact/fence checks: M1 fence paths exist.
for f in .agent/milestone-files/EP-033-M1.txt .agent/node-contracts/EP-033.md \
         .agent/execplans/EP-033-web-dashboard-and-desktop.md apps/web/package.json; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-033 M1: ok"
