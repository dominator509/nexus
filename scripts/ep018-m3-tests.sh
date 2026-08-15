#!/usr/bin/env sh
# EP-018 M3 gate: run the nexus-skills schema-parity integration suite
# through the REAL cargo test machinery with a vacuity guard.
#
# M3 owns real dependency/transport integration: the canonical
# schemas/skills/ documents validated against the REAL Rust contract
# types and the REAL on-disk bundles with the REAL jsonschema validator
# (EP-010 M3 pattern). A zero-match ep018_integration invocation must
# fail the gate (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep018-m3-tests.log"
: > "$log"

if ! cargo test --locked -p nexus-skills ep018_integration >>"$log" 2>&1; then
  echo "EP-018 M3: FAIL - cargo test ep018_integration failed" >&2
  tail -30 "$log" >&2
  exit 1
fi

# Vacuity guard 1: at least one test binary ran tests.
if ! grep -qE 'running [1-9][0-9]* tests' "$log"; then
  echo "EP-018 M3: FAIL - no tests matched ep018_integration (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# Vacuity guard 2: the run reports a passing result with a non-zero
# count (a filtered-out zero-count pass must fail the gate).
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-018 M3: FAIL - no passing ep018_integration tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-018 M3: ok"
