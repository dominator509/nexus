#!/usr/bin/env sh
# RX-019 remediation battery: EP-019 IncidentEngine + EP-038 GlitchTip truth
# (AUD-017 production IncidentEngine; AUD-055 GlitchTip https:// DSNs sent
#  over plaintext; AUD-056 correlation/trace context stripped before
#  GlitchTip delivery; AUD-057 GlitchTip-outage quarantine process-local)
#
# The battery runs the REAL gates that own each repair surface:
#   - AUD-017 (M1)  -> EP-019 gates M1-M4 (nexus-healing engine contract,
#     incident workflow vitest, healing integration pytest, failure suite)
#   - AUD-055/056 (M2/M3) -> EP-038 M3 gate (real ephemeral GlitchTip 6.1.8
#     fixture, production adapter unit suite, real-provider integration,
#     stopped-provider phase) + EP-038 M4 gate (ops runtime unit suite)
#   - AUD-057 (M4)  -> EP-038 M4 gate (real provider failure suite +
#     ops runtime unit suite; durable quarantine proofs live in the ops lib)
# plus clippy/fmt on the touched crate surface.
set -eu
cd "$(dirname "$0")/.."
. ./scripts/env.sh

pass=0
fail=0
note() { echo "ok - $1"; pass=$((pass + 1)); }
bad() { echo "FAIL - $1"; fail=$((fail + 1)); }

# --- M1: AUD-017 production IncidentEngine (EP-019 gates) ---
for g in ep019-m1-tests.sh ep019-m2-tests.sh ep019-m3-tests.sh ep019-m4-tests.sh; do
  if timeout 500 sh "scripts/$g" >"/tmp/rx019-$g.log" 2>&1; then
    note "$g (AUD-017 IncidentEngine)"
  else
    bad "$g"
    tail -15 "/tmp/rx019-$g.log"
  fi
done

# --- M2/M3: AUD-055 https TLS + AUD-056 correlation context (EP-038 M3 gate) ---
if timeout 550 sh scripts/ep038-m3-tests.sh >/tmp/rx019-ep038-m3.log 2>&1; then
  note "ep038-m3-tests.sh (AUD-055 https TLS + AUD-056 correlation context)"
else
  bad "ep038-m3-tests.sh"
  tail -20 /tmp/rx019-ep038-m3.log
fi

# --- M3/M4: AUD-056 correlation + AUD-057 durable quarantine (EP-038 M4 gate) ---
if timeout 550 sh scripts/ep038-m4-tests.sh >/tmp/rx019-ep038-m4.log 2>&1; then
  note "ep038-m4-tests.sh (AUD-056 correlation + AUD-057 durable quarantine)"
else
  bad "ep038-m4-tests.sh"
  tail -20 /tmp/rx019-ep038-m4.log
fi

# --- clippy + fmt on the touched crate surface ---
if timeout 300 cargo clippy -p nexus-healing -p nexus-glitchtip \
  -p nexus-observability-ops -p nexus-observability \
  --all-targets --locked -- -D warnings >/tmp/rx019-clippy.log 2>&1; then
  note "clippy -D warnings clean"
else
  bad "clippy -D warnings"
  tail -15 /tmp/rx019-clippy.log
fi

if timeout 120 cargo fmt -p nexus-healing -p nexus-glitchtip \
  -p nexus-observability-ops -p nexus-observability \
  -- --check >/tmp/rx019-fmt.log 2>&1; then
  note "cargo fmt clean"
else
  bad "cargo fmt"
  tail -15 /tmp/rx019-fmt.log
fi

echo
echo "RX-019 battery: $pass ok, $fail fail"
[ "$fail" -eq 0 ]
