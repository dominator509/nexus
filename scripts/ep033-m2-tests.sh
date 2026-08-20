#!/usr/bin/env sh
# EP-033 M2 gate: run the @nexus/desktop shell core-behavior suite
# through the REAL pnpm/vitest machinery with vacuity guards, plus the
# M1 regression (desktop shares the web contracts, so the web contract
# suite must stay green).
#
# The M2 changed-file fence is apps/desktop/ (plus pnpm-workspace.yaml
# and pnpm-lock.yaml for the new importer), so the authoritative gate
# is the desktop suite (tsc --noEmit + vitest ep033_unit_*) plus a
# dependency-direction proof and the M1 regression. Vacuity guards are
# required: `vitest -t <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class).
set -eu
export CI=true

log="/tmp/ep033-m2-tests.log"
: > "$log"

fail() {
  echo "EP-033 M2 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-033 M2 gate: $1"; }

PNPM="${PNPM_BIN:-/root/.local/share/mise/installs/pnpm/11.17.0/pnpm}"

# Vacuity guard 0: the desktop package must exist.
if [ ! -f apps/desktop/package.json ]; then
  fail "apps/desktop/package.json missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in \
  src/index.ts \
  src/runtime.ts \
  src/dispatcher.ts \
  src/approvals.ts \
  src/viewstate.ts \
  src/prefs.ts; do
  if [ ! -f "apps/desktop/$f" ]; then
    fail "apps/desktop/$f missing"
  fi
done
ok "desktop package and sources present"

# Real typecheck: tsc --noEmit must pass (compile/typecheck gate).
if ! (cd apps/desktop && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Real test run: the full desktop ep033_unit suite through vitest.
if ! (cd apps/desktop && "$PNPM" exec vitest run src/__tests__ >>"$log" 2>&1); then
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

# Vacuity guard 4: the desktop dependency-direction test ran.
if ! grep -q 'ep033_unit_desktop_dependency_direction' "$log"; then
  fail "desktop dependency-direction test did not run" "$log"
fi

# Vacuity guard 5 (anti-masking): an EP-033-owned desktop test must be
# observed. This fails if the gate accidentally executes only a prior
# node's or the web package's tests.
if ! grep -q 'ep033_unit_desktop_runtime' "$log"; then
  fail "EP-033-owned desktop runtime test did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep033_unit_desktop_dispatcher' "$log"; then
  fail "EP-033-owned desktop dispatcher test did not run (anti-masking guard)" "$log"
fi

total=$(grep -oE 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | awk '{s+=$2} END {print s}')
ok "real desktop suite passed (${total} tests total)"

# M1 regression: the desktop shares @nexus/web contracts; the web
# contract suite must remain green.
if ! sh scripts/ep033-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression (web contract suite) failed" "$log"
fi
ok "M1 regression green"

# Milestone artifact/fence checks.
for f in .agent/milestone-files/EP-033-M2.txt .agent/node-contracts/EP-033.md \
         .agent/execplans/EP-033-web-dashboard-and-desktop.md apps/desktop/package.json; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-033 M2: ok"
