#!/usr/bin/env sh
# EP-033 M3 gate: run the @nexus/ui shared component suite through the
# REAL pnpm/vitest machinery (React 19 real server rendering) with
# vacuity guards, plus M1/M2 regressions.
#
# The M3 changed-file fence is packages/ui/ (plus COMPONENT_REGISTRY
# for the React entry), so the authoritative gate is the ui suite
# (tsc --noEmit + vitest ep033_integration_* + ep033_unit_ui_*) plus
# dependency-direction and the M1/M2 regressions. Vacuity guards are
# required: `vitest -t <filter>` exits 0 on a zero-match filter
# (EP-001 gate-masking class).
set -eu
export CI=true

log="/tmp/ep033-m3-tests.log"
: > "$log"

fail() {
  echo "EP-033 M3 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-033 M3 gate: $1"; }

PNPM="${PNPM_BIN:-/root/.local/share/mise/installs/pnpm/11.17.0/pnpm}"

# Vacuity guard 0: the ui package must exist.
if [ ! -f packages/ui/package.json ]; then
  fail "packages/ui/package.json missing"
fi

# Vacuity guard 0b: the owned production sources must exist.
for f in \
  src/index.ts \
  src/components/capability-button.tsx \
  src/components/approval-card-view.tsx \
  src/components/status-badge.tsx \
  src/components/chat-composer.tsx \
  src/components/dashboard-shell-view.tsx; do
  if [ ! -f "packages/ui/$f" ]; then
    fail "packages/ui/$f missing"
  fi
done
ok "ui package and sources present"

# Real typecheck: tsc --noEmit must pass (compile/typecheck gate).
if ! (cd packages/ui && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Real test run: the full ui suite through vitest.
if ! (cd packages/ui && "$PNPM" exec vitest run src/__tests__ >>"$log" 2>&1); then
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

# Vacuity guard 4: the ui dependency-direction test ran.
if ! grep -q 'ep033_unit_ui_dependency_direction' "$log"; then
  fail "ui dependency-direction test did not run" "$log"
fi

# Vacuity guard 5 (anti-masking): an EP-033-owned integration test must
# be observed, proving REAL React rendering ran (not a prior node's or
# a zero-match filter).
if ! grep -q 'ep033_integration_capability_gate' "$log"; then
  fail "EP-033-owned capability integration test did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep033_integration_approval_classes' "$log"; then
  fail "EP-033-owned approval integration test did not run (anti-masking guard)" "$log"
fi

total=$(grep -oE 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | awk '{s+=$2} END {print s}')
ok "real ui suite passed (${total} tests total)"

# Real React version pin: the registry entry must name React 19.2.8.
if ! grep -qi 'React 19.2.8' COMPONENT_REGISTRY.yaml; then
  fail "COMPONENT_REGISTRY.yaml missing React 19.2.8 entry"
fi
ok "component registry React 19.2.8 entry present"

# M1/M2 regressions: the ui package consumes @nexus/web contracts.
if ! sh scripts/ep033-m1-tests.sh >>"$log" 2>&1; then
  fail "M1 regression (web contract suite) failed" "$log"
fi
if ! sh scripts/ep033-m2-tests.sh >>"$log" 2>&1; then
  fail "M2 regression (desktop suite) failed" "$log"
fi
ok "M1/M2 regressions green"

# Milestone artifact/fence checks.
for f in .agent/milestone-files/EP-033-M3.txt .agent/node-contracts/EP-033.md \
         .agent/execplans/EP-033-web-dashboard-and-desktop.md packages/ui/package.json; do
  if [ ! -f "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-033 M3: ok"
