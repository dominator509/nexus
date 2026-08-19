#!/usr/bin/env sh
# EP-026 M2 gate: run the nexus-gmail adapter suite through the REAL
# cargo test machinery with a vacuity guard.
#
# The M2 changed-files fence is connectors/gmail/ (Rust crate
# nexus-gmail), so the authoritative gate is the ep026_unit cargo
# suite of that crate. The vacuity guard is required: `cargo test
# <filter>` exits 0 on a zero-match filter (EP-001 gate-masking
# class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep026-m2-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-gmail ep026_unit >>"$log" 2>&1; then
  echo "EP-026 M2: FAIL - cargo test nexus-gmail ep026_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard 1: at least one test binary ran tests.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-026 M2: FAIL - no tests matched ep026_unit (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the run reports a passing result with a non-zero
# count.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-026 M2: FAIL - no passing ep026_unit tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-026 M2: ok"
