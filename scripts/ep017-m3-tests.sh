#!/usr/bin/env sh
# EP-017 M3 gate: run the agent workflow integration suite through the
# REAL vitest + @nexus/workflows machinery with a vacuity guard.
#
# The M3 changed-files fence is packages/workflows/src/agents/ (TS), so
# the authoritative gate is the ep017_integration vitest suite, not a
# cargo test on nexus-agents (EP-001 gate-masking class: the old M3 gate
# ran `cargo test -p nexus-agents ep017_integration` which matched zero
# tests and passed vacuously).
set -eu
. scripts/env.sh
export NO_COLOR=1

log="/tmp/ep017-m3-vitest.log"
: > "$log"

if ! pnpm --filter @nexus/workflows exec vitest run -t ep017_integration >>"$log" 2>&1; then
  echo "EP-017 M3: FAIL - vitest ep017_integration failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard: vitest exits 0 on a zero-match name filter.
if ! sed 's/\x1b\[[0-9;]*m//g' "$log" | grep -qE 'Tests[[:space:]]+[1-9][0-9]* passed'; then
  echo "EP-017 M3: FAIL - no passing tests matched ep017_integration (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Typecheck the agents module with the real tsc (provider-neutral
# contracts must still compile against the real workflow machinery).
if ! pnpm --filter @nexus/workflows exec tsc --noEmit >>"$log" 2>&1; then
  echo "EP-017 M3: FAIL - tsc --noEmit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-017 M3: ok"
