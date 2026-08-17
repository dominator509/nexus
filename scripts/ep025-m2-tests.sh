#!/usr/bin/env sh
# EP-025 M2 gate: run the nexus-asterisk production adapter core suite
# through the REAL cargo test machinery with a vacuity guard.
#
# The M2 changed-files fence is connectors/asterisk/ (Rust crate
# nexus-asterisk), so the authoritative gate is the ep025_unit cargo
# suite of that crate. The vacuity guard is required: `cargo test
# <filter>` exits 0 on a zero-match filter (EP-001 gate-masking
# class). The pre-created M2 gate reran the M1 contract suite
# (nexus-telephony) - masking class; replaced here.
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep025-m2-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-asterisk ep025_unit >>"$log" 2>&1; then
  echo "EP-025 M2: FAIL - cargo test ep025_unit (nexus-asterisk) failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard 1: at least one test binary ran tests.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-025 M2: FAIL - no tests matched ep025_unit (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the run reports a passing result with a non-zero
# count.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-025 M2: FAIL - no passing ep025_unit tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 3: the adapter core tests (not just the contract
# crate) actually ran.
if ! grep -qE 'ep025_unit_(state_mapping|originate|capability_gate|answer_verification|idempotency|exact_target|availability|hangup)' "$log"; then
  echo "EP-025 M2: FAIL - adapter core tests did not run (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-025 M2: ok"
