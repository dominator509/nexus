#!/usr/bin/env sh
# EP-044 smoke gate ownership regression (owner GraphLock amendment 2026-08-14).
# Proves the runtime smoke activation contract:
#   1. stage < runtime owner  -> runtime smoke is NOT invoked (not-applicable)
#   2. activation wiring points at the runtime owner (EP-044), not EP-012
#   3. stage >= runtime owner + runtime absent -> the smoke FAILS (fail-closed)
set -eu
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

fail() { echo "smoke gate regression: FAIL - $*" >&2; exit 1; }

# 1. Before the runtime owner is DONE, the runtime smoke must NOT be invoked.
out=$(sh scripts/smoke-test.sh 2>&1 || true)
echo "$out" | grep -q "runtime smoke: not-applicable-before EP-044" \
  || fail "expected not-applicable sentinel, got: $out"
echo "$out" | grep -q "smoke test: ok" \
  || fail "smoke stage must stay green while not applicable, got: $out"
if echo "$out" | grep -q "runtime smoke: ok"; then
  fail "runtime smoke ran before owner DONE (must be not-applicable)"
fi

# 2. Activation wiring must reference the runtime owner.
grep -q 'runtime_owner="EP-044"' scripts/smoke-test.sh \
  || fail "smoke-test.sh does not activate at EP-044"

# 3. Once the owner is DONE the smoke is mandatory and fails closed when the
#    runtime is absent: point at a dead address and require failure.
if NEXUS_SMOKE_URL=http://127.0.0.1:1 sh scripts/smoke/runtime.sh >/dev/null 2>&1; then
  fail "runtime smoke passed with no server (must fail closed)"
fi

echo "smoke gate regression: ok"
