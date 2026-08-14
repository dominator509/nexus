#!/usr/bin/env sh
set -eu
# EP-010 M5 vacuity guard (directive S): the live-fire pytest selection
# must actually RUN tests. pytest exits 0 even when the function filter
# matches zero tests ("no tests ran"), which would let a vacuous gate
# print green. This script captures the run, requires a non-zero
# passing test count, and fails otherwise.
. scripts/env.sh
export NO_COLOR=1
log=/tmp/ep010-livefire-pytest.log
rm -f "$log"
if ! uv run --frozen pytest tests/capabilities -q --tb=native \
  -o python_functions="ep010_livefire_*" >"$log" 2>&1; then
  echo "EP-010 vacuity: FAIL - live-fire pytest did not pass:" >&2
  cat "$log" >&2
  exit 1
fi
# Require a real non-zero test count, e.g. "7 passed in 3.20s".
if ! grep -Eq '^[0-9]+ passed' "$log"; then
  echo "EP-010 vacuity: FAIL - no tests ran (vacuity guard):" >&2
  cat "$log" >&2
  exit 1
fi
count=$(grep -Eo '^[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 1 ]; then
  echo "EP-010 vacuity: FAIL - zero tests passed" >&2
  cat "$log" >&2
  exit 1
fi
echo "EP-010 vacuity: ok ($count live-fire tests passed)"
