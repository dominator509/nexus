#!/usr/bin/env sh
# EP-022 M1 gate: run the nexus-audio contract suite through real cargo
# with a vacuity guard.
#
# The pre-created node script M1 entry was artifact-only (EP-001
# gate-masking class); this gate runs the real ep022_unit suite and
# fails closed when no test ran.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep022-m1-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-audio ep022_unit >>"$log" 2>&1; then
  echo "EP-022 M1: FAIL - cargo test nexus-audio ep022_unit failed" >&2
  tail -40 "$log" >&2
  exit 1
fi

# Vacuity guard: at least one ep022_unit test passed.
if ! grep -qE '^test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-022 M1: FAIL - no ep022_unit tests passed (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -3 "$log"
echo "EP-022 M1: ok"
