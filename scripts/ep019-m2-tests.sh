#!/usr/bin/env sh
# EP-019 M2 gate: run the incident workflow integration suite through
# the REAL vitest + @nexus/workflows machinery with a vacuity guard.
#
# The M2 changed-files fence is packages/workflows/src/incidents/ (TS),
# so the authoritative gate is the ep019_integration vitest suite, not a
# cargo test (EP-001 gate-masking class: a zero-match filter exits 0).
set -eu
. scripts/env.sh
export NO_COLOR=1

log="/tmp/ep019-m2-vitest.log"
: > "$log"

if ! pnpm --filter @nexus/workflows exec vitest run -t ep019_integration >>"$log" 2>&1; then
  echo "EP-019 M2: FAIL - vitest ep019_integration failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard: vitest exits 0 on a zero-match name filter.
if ! sed 's/\x1b\[[0-9;]*m//g' "$log" | grep -qE 'Tests[[:space:]]+[1-9][0-9]* passed'; then
  echo "EP-019 M2: FAIL - no passing tests matched ep019_integration (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Typecheck the incidents module with the real tsc (provider-neutral
# contracts must still compile against the real workflow machinery).
if ! pnpm --filter @nexus/workflows exec tsc --noEmit >>"$log" 2>&1; then
  echo "EP-019 M2: FAIL - tsc --noEmit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-019 M2: ok"
