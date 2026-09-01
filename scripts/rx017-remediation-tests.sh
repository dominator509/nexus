#!/usr/bin/env sh
# RX-017 remediation battery: onboarding/provisioning truth
# (AUD-042 Setup/secure-enrollment product via EP-035 gates + LF-001 live-fire;
#  AUD-043 enrollment claim binds bootstrap secret; AUD-044 owner ladder at the
#  durable boundary; AUD-045 AMBIGUOUS+RECONCILED retry requires negative
#  observation; AUD-046 GenericSshProvider real transport; AUD-047 placement
#  requires observed capacity + health; AUD-048 cost ceiling enforced)
#
# The battery runs the REAL test suites that prove each milestone plus the
# workspace gates. LF-001 evidence is refreshed by the ep035 M5 gate.
set -eu
cd "$(dirname "$0")/.."
export CI=true
export GIT_TERMINAL_PROMPT=0
export GIT_PAGER=cat
export PAGER=cat
export DEBIAN_FRONTEND=noninteractive
export CARGO_TERM_COLOR=never

pass=0
fail=0
note() { echo "ok - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }

# --- M1: AUD-042 Setup/secure-enrollment product (EP-035 gates) ---
for g in ep035-m1-tests.sh ep035-m2-tests.sh ep035-m3-tests.sh ep035-m4-tests.sh ep035-m5-tests.sh; do
  if timeout 500 sh "scripts/$g" >"/tmp/rx017-$g.log" 2>&1; then
    note "$g (AUD-042 setup product)"
  else
    bad "$g"
    tail -15 "/tmp/rx017-$g.log"
  fi
done

# --- M2: AUD-043 enrollment claim secret-possession (onboarding) ---
out=$(pnpm --filter @nexus/onboarding exec vitest run src/__tests__/integration/enrollment-token.integration.test.ts 2>&1 || true)
if echo "$out" | grep -q "Test Files.*passed\|Tests.*passed" && ! echo "$out" | grep -q "failed"; then
  note "onboarding enrollment integration (AUD-043)"
else
  bad "onboarding enrollment integration"
  echo "$out" | tail -20
fi

# --- M3: AUD-044 owner ladder durable boundary (onboarding) ---
out=$(pnpm --filter @nexus/onboarding exec vitest run src/__tests__/integration/owner-bootstrap.integration.test.ts 2>&1 || true)
if echo "$out" | grep -q "Test Files.*passed\|Tests.*passed" && ! echo "$out" | grep -q "failed"; then
  note "onboarding owner-bootstrap integration (AUD-044)"
else
  bad "onboarding owner-bootstrap integration"
  echo "$out" | tail -20
fi

# --- M4: AUD-045 recovery retry negative-observation (onboarding + setup) ---
out=$(pnpm --filter @nexus/onboarding exec vitest run src/__tests__/integration/recovery-flow.integration.test.ts 2>&1 || true)
if echo "$out" | grep -q "Test Files.*passed\|Tests.*passed" && ! echo "$out" | grep -q "failed"; then
  note "onboarding recovery integration (AUD-045)"
else
  bad "onboarding recovery integration"
  echo "$out" | tail -20
fi
out=$(pnpm --filter @nexus/setup exec vitest run src/__tests__/ep035_unit_recovery.test.ts 2>&1 || true)
if echo "$out" | grep -q "Test Files.*passed\|Tests.*passed" && ! echo "$out" | grep -q "failed"; then
  note "setup recovery contract (AUD-045)"
else
  bad "setup recovery contract"
  echo "$out" | tail -20
fi

# --- M5: AUD-046 GenericSshProvider real transport (existing-ssh) ---
if timeout 500 cargo test -p nexus-provider-existing-ssh --locked 2>/tmp/rx017-existing-ssh.log; then
  note "existing-ssh real transport (AUD-046)"
else
  bad "existing-ssh real transport"
  tail -15 /tmp/rx017-existing-ssh.log
fi

# --- M6/M7: AUD-047/048 placement (nexus-compute + EP-036 gates) ---
if timeout 500 cargo test -p nexus-compute --locked 2>/tmp/rx017-compute.log; then
  note "nexus-compute placement (AUD-047/048)"
else
  bad "nexus-compute placement"
  tail -15 /tmp/rx017-compute.log
fi
# EP-036 M2 gate needs OpenTofu via mise env (SPEC-016).
eval "$(mise env 2>/dev/null || true)"
for g in ep036-m1-tests.sh ep036-m2-tests.sh ep036-m3-tests.sh ep036-m4-tests.sh ep036-m5-tests.sh; do
  if timeout 500 sh "scripts/$g" >"/tmp/rx017-$g.log" 2>&1; then
    note "$g (EP-036 provisioning gates)"
  else
    bad "$g"
    tail -15 "/tmp/rx017-$g.log"
  fi
done

echo
echo "RX-017 battery: $pass ok, $fail fail"
[ "$fail" -eq 0 ]
