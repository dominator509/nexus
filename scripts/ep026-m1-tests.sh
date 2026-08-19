#!/usr/bin/env sh
# EP-026 M1 gate: run the nexus-email contract suite through the REAL
# cargo test machinery with a vacuity guard.
#
# The M1 changed-files fence is crates/nexus-email/ + tests/email/
# (Rust crates nexus-email, nexus-email-e2e), so the authoritative
# gate is the ep026_unit cargo suite of the contract crate plus the
# e2e package surface tests. The vacuity guard is required: `cargo
# test <filter>` exits 0 on a zero-match filter (EP-001 gate-masking
# class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep026-m1-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-email ep026_unit >>"$log" 2>&1; then
  echo "EP-026 M1: FAIL - cargo test nexus-email ep026_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

if ! cargo test --locked -p nexus-email-e2e ep026_unit >>"$log" 2>&1; then
  echo "EP-026 M1: FAIL - cargo test nexus-email-e2e ep026_unit failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard 1: at least one test binary ran tests.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-026 M1: FAIL - no tests matched ep026_unit (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the run reports a passing result with a non-zero
# count.
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-026 M1: FAIL - no passing ep026_unit tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-026 M1: ok"
