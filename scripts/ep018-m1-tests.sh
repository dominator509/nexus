#!/usr/bin/env sh
# EP-018 M1 gate: run the nexus-skills contract suite through the REAL
# cargo test machinery with a vacuity guard.
#
# The M1 changed-files fence is crates/nexus-skills/ (Rust), so the
# authoritative gate is the ep018_unit cargo suite. The vacuity guard is
# required: `cargo test <filter>` exits 0 on a zero-match filter (EP-001
# gate-masking class), so a zero-match invocation must fail the gate.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep018-m1-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-skills ep018_unit >>"$log" 2>&1; then
  echo "EP-018 M1: FAIL - cargo test ep018_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard 1: at least one test binary ran tests.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-018 M1: FAIL - no tests matched ep018_unit (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the run reports a passing result with a non-zero
# count (a filtered-out zero-count pass must fail the gate).
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-018 M1: FAIL - no passing ep018_unit tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-018 M1: ok"
