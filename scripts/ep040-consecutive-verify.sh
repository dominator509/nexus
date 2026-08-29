#!/usr/bin/env sh
# AUD-062 gate: EP-040 must enforce three consecutive full verifies.
#
# The canonical ConsecutiveVerify policy (nexus-test-execution) already
# models the rule; this gate feeds it REAL results. The full canonical
# verification ladder (scripts/verify.sh) is executed three times. Each
# run's result (GREEN only when the exact sentinel is observed) is fed
# into ConsecutiveVerify::new(3); the gate passes only when the policy
# reports the sequence complete.
#
# This runs the full ladder three times on purpose: a flake that passes
# once but not three times in a row is exactly what AUD-062 demands be
# caught. No retry, no best-of, no counting a partial run.
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

log="/tmp/ep040-consecutive-verify.log"
: > "$log"

fail() { echo "EP-040 consecutive-verify gate: FAIL - $1" >&2; exit 1; }

# The harness binary is built from the canonical policy crate.
if ! cargo build -p nexus-test-execution --bin consecutive_verify_gate --locked >> "$log" 2>&1; then
  fail "cannot build consecutive_verify_gate harness"
fi

results=""
i=1
while [ "$i" -le 3 ]; do
  echo "=== full verify run $i/3 ===" >> "$log"
  if sh scripts/verify.sh >> "$log" 2>&1 && grep -q "^verify: ok$" "$log"; then
    echo "run $i: GREEN" >> "$log"
    results="$results
GREEN"
  else
    echo "run $i: RED" >> "$log"
    results="$results
RED"
  fi
  i=$((i + 1))
done

bin=$(find target/debug -maxdepth 1 -name 'consecutive_verify_gate' -type f | head -n 1)
[ -n "$bin" ] || fail "consecutive_verify_gate binary missing"

if ! printf '%s\n' "$results" | "$bin" >> "$log" 2>&1; then
  tail -20 "$log" >&2
  fail "three consecutive full verifies NOT observed (AUD-062)"
fi
echo "EP-040 consecutive-verify gate: ok (3/3 full verifies green)"
