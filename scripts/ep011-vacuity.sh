#!/usr/bin/env sh
set -eu
# EP-011 M3 vacuity guard (directive U): the M3 pytest selection must
# actually RUN tests. pytest exits 0 even when the function filter
# matches zero tests ("no tests ran"), which would let a vacuous gate
# print green. This script captures the run, requires a non-zero
# passing test count, and fails otherwise.
. scripts/env.sh
export NO_COLOR=1
log=/tmp/ep011-m3-pytest.log
rm -f "$log"
if ! uv run --frozen pytest tests/connectors -q --tb=native \
  -o python_functions="ep011_integration_* ep011_failure_*" >"$log" 2>&1; then
  echo "EP-011 vacuity: FAIL - M3 pytest did not pass:" >&2
  cat "$log" >&2
  exit 1
fi
# Require a real non-zero test count, e.g. "58 passed in 20.13s".
if ! grep -Eq '^[0-9]+ passed' "$log"; then
  echo "EP-011 vacuity: FAIL - no tests ran (vacuity guard):" >&2
  cat "$log" >&2
  exit 1
fi
count=$(grep -Eo '^[0-9]+ passed' "$log" | grep -Eo '[0-9]+' | head -1)
if [ "${count:-0}" -lt 1 ]; then
  echo "EP-011 vacuity: FAIL - zero tests passed" >&2
  cat "$log" >&2
  exit 1
fi
echo "EP-011 vacuity: ok ($count M3 tests passed)"
