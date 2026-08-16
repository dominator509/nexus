#!/usr/bin/env sh
# EP-018 M4 gate: run the nexus-skills forced-failure suite through the
# REAL cargo test machinery with a vacuity guard.
#
# M4 owns forced failures, abuse cases, and observability. The gate
# runs the ep018_failure suite (real mechanisms, no mocks of the proven
# component) and fails closed if no test matches the filter (EP-001
# gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep018-m4-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-skills ep018_failure >>"$log" 2>&1; then
  echo "EP-018 M4: FAIL - cargo test ep018_failure failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard 1: at least one test binary ran tests.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-018 M4: FAIL - no tests matched ep018_failure (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the run reports a passing result with a non-zero
# count (a filtered-out zero-count pass must fail the gate).
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-018 M4: FAIL - no passing ep018_failure tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-018 M4: ok"
