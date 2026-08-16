#!/usr/bin/env sh
# EP-022 M2 gate: run the nexus-assist-satellite adapter suite through
# the REAL cargo test machinery with a vacuity guard.
#
# The M2 changed-files fence is connectors/assist-satellite/ (Rust crate
# nexus-assist-satellite), so the authoritative gate is the ep022_unit
# cargo suite of that crate. The vacuity guard is required: `cargo test
# <filter>` exits 0 on a zero-match filter (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep022-m2-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-assist-satellite ep022_unit >>"$log" 2>&1; then
  echo "EP-022 M2: FAIL - cargo test ep022_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard 1: at least one test binary ran tests.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-022 M2: FAIL - no tests matched ep022_unit (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the run reports a passing result with a non-zero
# count.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-022 M2: FAIL - no passing ep022_unit tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-022 M2: ok"
