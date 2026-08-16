#!/usr/bin/env sh
# EP-020 M4 gate: forced failures, abuse cases, and observability.
#
# The M4 changed-files fence is tests/home/ (real-container failure
# suite + operations README) plus the contract-crate forced-failure
# suite (crates/nexus-home/tests/ep020_failure_forced.rs). The
# authoritative gate runs BOTH through real machinery:
#   1. cargo test -p nexus-home ep020_failure  (contract fail-closed)
#   2. pytest  tests/home/test_ep020_failure_home_assistant.py
#      (real HA container failure mechanisms)
# Vacuity guards are required: cargo and pytest both exit 0 on a
# zero-match run (EP-001 gate-masking class).
set -eu
export CI=true
export CARGO_TERM_COLOR=never

log="/tmp/ep020-m4-tests.log"
: > "$log"

# 1. Contract-crate forced-failure suite (Rust).
if ! cargo test --locked -p nexus-home ep020_failure >>"$log" 2>&1; then
  echo "EP-020 M4: FAIL - cargo test ep020_failure failed" >&2
  tail -40 "$log" >&2
  exit 1
fi
if ! grep -qE 'test result: ok\. [1-9][0-9]* passed' "$log"; then
  echo "EP-020 M4: FAIL - no passing ep020_failure contract tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

# 2. Real-container failure suite (Python; system python3 carries
#    websocket-client + pytest, EP-011 sidecar precedent). --tb=native:
#    pytest 9 dumps helper-frame locals (secrets) in default tracebacks.
if ! python3 -m pytest tests/home/test_ep020_failure_home_assistant.py -q --tb=native >>"$log" 2>&1; then
  echo "EP-020 M4: FAIL - pytest failure suite failed" >&2
  tail -40 "$log" >&2
  exit 1
fi
if ! grep -qE '^[0-9]+ passed' "$log"; then
  echo "EP-020 M4: FAIL - no passing pytest failure tests (vacuity guard)" >&2
  tail -10 "$log" >&2
  exit 1
fi

tail -6 "$log"
echo "EP-020 M4: ok"
