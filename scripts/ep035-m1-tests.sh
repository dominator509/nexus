#!/usr/bin/env sh
# EP-035 M1 gate: run the @nexus/setup contract suite through the REAL
# pnpm/vitest machinery with vacuity guards.
#
# The M1 changed-file fence is apps/setup/ (contract package) plus the
# node script and plan files, so the authoritative gate is the setup
# contract suite (tsc --noEmit + vitest ep035_unit_*) plus dependency
# direction and schema parity proofs. Vacuity guards are required:
# `vitest -t <filter>` exits 0 on a zero-match filter (EP-001
# gate-masking class), so a green M1 must observe a real non-zero
# passing count, the owned proof names, and zero skipped/failed tests.
set -eu
export CI=true

log="/tmp/ep035-m1-tests.log"
: > "$log"

fail() {
  echo "EP-035 M1 gate: FAIL - $1" >&2
  tail -40 "${2:-/dev/null}" >&2 2>/dev/null || true
  exit 1
}
ok() { echo "EP-035 M1 gate: $1"; }

PNPM="${PNPM_BIN:-pnpm}"

# Vacuity guard 0: the setup package must exist.
if [ ! -f apps/setup/package.json ]; then
  fail "apps/setup/package.json missing"
fi

# Vacuity guard 0b: the owned production contract sources must exist.
for f in \
  package.json \
  tsconfig.json \
  src/index.ts \
  src/contracts/errors.ts \
  src/contracts/validate.ts \
  src/contracts/wizard.ts \
  src/contracts/deployment.ts \
  src/contracts/hardware.ts \
  src/contracts/owner.ts \
  src/contracts/enrollment.ts \
  src/contracts/discovery.ts \
  src/contracts/integration.ts \
  src/contracts/recovery.ts; do
  if [ ! -f "apps/setup/$f" ]; then
    fail "apps/setup/$f missing"
  fi
done
ok "setup contract package and sources present"

# Real typecheck: tsc --noEmit must pass (compile/typecheck gate).
if ! (cd apps/setup && "$PNPM" exec tsc --noEmit >>"$log" 2>&1); then
  fail "tsc --noEmit failed" "$log"
fi
ok "tsc --noEmit clean"

# Real test run: the full ep035_unit suite through vitest.
if ! (cd apps/setup && "$PNPM" exec vitest run src/__tests__ >>"$log" 2>&1); then
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

# Vacuity guard 4: the dependency-direction proof ran and passed.
if ! grep -q 'ep035_unit_dependency_direction' "$log"; then
  fail "dependency-direction test did not run" "$log"
fi

# Vacuity guard 5: schema parity proof executed.
if ! grep -q 'ep035_unit_schema_parity' "$log"; then
  fail "schema-parity test did not run" "$log"
fi

# Vacuity guard 6: deny-unknown proof executed.
if ! grep -q 'ep035_unit_validate' "$log"; then
  fail "deny-unknown validation test did not run" "$log"
fi

# Vacuity guard 7: state/invariant proof executed (wizard transitions).
if ! grep -q 'ep035_unit_wizard' "$log"; then
  fail "wizard state/invariant test did not run" "$log"
fi

# Vacuity guard 8: secret redaction proof executed (enrollment).
if ! grep -q 'ep035_unit_enrollment' "$log"; then
  fail "enrollment secret-redaction test did not run" "$log"
fi

# Vacuity guard 9 (anti-masking): EP-035-owned proof names must be
# observed; this fails if the gate accidentally executes only a prior
# node's tests or a zero-match filter.
if ! grep -q 'ep035_unit_surfaces' "$log"; then
  fail "EP-035-owned surface test did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep035_unit_deployment' "$log"; then
  fail "EP-035-owned deployment contract test did not run (anti-masking guard)" "$log"
fi
if ! grep -q 'ep035_unit_owner' "$log"; then
  fail "EP-035-owned owner bootstrap test did not run (anti-masking guard)" "$log"
fi

total=$(grep -oE 'Tests[[:space:]]+[1-9][0-9]* passed' "$log" | awk '{s+=$2} END {print s}')
ok "real suite passed (${total} tests total)"

# Milestone artifact/fence checks: M1 fence paths exist.
for f in .agent/milestone-files/EP-035-M1.txt .agent/node-contracts/EP-035.md \
         .agent/execplans/EP-035-setup-wizard-and-onboarding.md apps/setup/package.json; do
  if [ ! -e "$f" ]; then
    fail "$f missing"
  fi
done
ok "milestone fence and ownership artifacts present"

echo "EP-035 M1: ok"
