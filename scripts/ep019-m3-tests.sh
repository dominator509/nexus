#!/usr/bin/env sh
# EP-019 M3 gate: run the self-healing integration suite through the
# REAL pytest machinery with a vacuity guard.
#
# The M3 changed-files fence is tests/healing/ (repo-level integration
# suite), so the authoritative gate is the ep019_integration pytest
# selection, not a cargo test (EP-001 gate-masking class: a zero-match
# filter exits 0).
set -eu
. scripts/env.sh
export NO_COLOR=1

log="/tmp/ep019-m3-pytest.log"
: > "$log"

if ! uv run --frozen pytest tests/healing -q --tb=native \
  -o python_functions="ep019_integration_*" >>"$log" 2>&1; then
  echo "EP-019 M3: FAIL - pytest ep019_integration failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard: pytest exits 0 even when the function filter matches
# zero tests; require a real non-zero passing count.
if ! grep -Eq '^[0-9]+ passed' "$log"; then
  echo "EP-019 M3: FAIL - no tests ran (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

count=$(grep -Eo '^[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 1 ]; then
  echo "EP-019 M3: FAIL - zero tests passed (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-019 M3: ok"
